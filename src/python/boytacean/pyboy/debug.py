from enum import Enum
from typing import Any, Callable, Dict, List, Tuple, Union

from ..gb import GameBoy


class StandardComparisonType(Enum):
    EXACT = 1
    LESS_THAN = 2
    GREATER_THAN = 3
    LESS_THAN_OR_EQUAL = 4
    GREATER_THAN_OR_EQUAL = 5


class DynamicComparisonType(Enum):
    UNCHANGED = 1
    CHANGED = 2
    INCREASED = 3
    DECREASED = 4
    MATCH = 5


class ScanMode(Enum):
    INT = 1
    BCD = 2


class SymbolTable:
    def __init__(self):
        self._symbols: Dict[str, Tuple[int, int]] = {}

    def load(self, path: str):
        with open(path, "r", encoding="utf-8", errors="replace") as handle:
            self._symbols.update(parse_symbols(handle.read()))

    def lookup(self, symbol: str) -> Tuple[int, int]:
        if symbol not in self._symbols:
            raise ValueError(f"Unknown symbol: {symbol}")
        return self._symbols[symbol]

    def __contains__(self, symbol: str) -> bool:
        return symbol in self._symbols

    def __len__(self) -> int:
        return len(self._symbols)


class HookRegistry:
    """
    Lightweight pure-Python hook manager. Hooks fire when the
    program counter matches a registered address, evaluated once per
    instruction step inside the run loop. There is no opcode patching
    — the cost when no hooks are registered is a single empty-dict
    check per frame, so the CPU hot path stays untouched.
    """

    def __init__(self, system: "GameBoy"):
        self._system = system
        self._hooks: Dict[int, Tuple[int, Callable, Any]] = {}

    def register(
        self,
        bank: int,
        addr: int,
        callback: Callable[[Any], None],
        context: Any = None,
    ):
        # bank is recorded for parity with the upstream API but is
        # ignored when matching, since the core does not yet expose
        # a per-bank PC predicate; addresses below 0x4000 always
        # belong to ROM bank 0, addresses in 0x4000..0x7FFF are bank
        # selected, the rest is bank-agnostic
        if addr in self._hooks:
            raise ValueError(
                f"Hook already registered for bank={bank} addr=0x{addr:04x}"
            )
        self._hooks[addr] = (bank, callback, context)

    def deregister(self, bank: int, addr: int):
        if addr not in self._hooks:
            raise ValueError(f"Hook not found for bank={bank} addr=0x{addr:04x}")
        del self._hooks[addr]

    def is_empty(self) -> bool:
        return not self._hooks

    def addresses(self) -> List[int]:
        return list(self._hooks.keys())

    def fire_for(self, pc: int):
        record = self._hooks.get(pc)
        if record is None:
            return
        _bank, callback, context = record
        callback(context)


class MemoryScanner:
    """
    Pure-Python implementation of the modern PyBoy memory scanner.
    Provides EXACT/LESS_THAN/GREATER_THAN comparisons for the initial
    scan and CHANGED/UNCHANGED/INCREASED/DECREASED/MATCH for follow
    up rescans, mirroring the upstream `pyboy.api.memory_scanner`
    semantics.
    """

    def __init__(self, system: "GameBoy"):
        self._system = system
        self._addresses: List[int] = []
        self._previous: Dict[int, int] = {}

    def scan_memory(
        self,
        target_value: Union[int, None] = None,
        start_addr: int = 0x0000,
        end_addr: int = 0xFFFF,
        standard_comparison_type: StandardComparisonType = StandardComparisonType.EXACT,
        value_type: ScanMode = ScanMode.INT,
        byte_width: int = 1,
        byteorder: str = "little",
    ) -> List[int]:
        if byte_width not in (1, 2, 4):
            raise ValueError(f"Unsupported byte_width: {byte_width}")
        if (
            standard_comparison_type == StandardComparisonType.EXACT
            and target_value is None
        ):
            raise ValueError("EXACT comparison requires a target_value")

        matches: List[int] = []
        snapshot: Dict[int, int] = {}
        for addr in range(start_addr, end_addr - byte_width + 2):
            value = self._read_value(addr, byte_width, value_type, byteorder)
            snapshot[addr] = value
            if target_value is None:
                matches.append(addr)
                continue
            if self._compare_standard(value, target_value, standard_comparison_type):
                matches.append(addr)
        self._addresses = matches
        self._previous = snapshot
        return matches

    def rescan_memory(
        self,
        new_value: Union[int, None] = None,
        dynamic_comparison_type: DynamicComparisonType = DynamicComparisonType.UNCHANGED,
        byteorder: str = "little",
    ) -> List[int]:
        if dynamic_comparison_type == DynamicComparisonType.MATCH and new_value is None:
            raise ValueError("MATCH comparison requires a new_value")

        survivors: List[int] = []
        for addr in self._addresses:
            current = self._system.read_memory(addr)
            previous = self._previous.get(addr, current)
            if self._compare_dynamic(
                current, previous, new_value, dynamic_comparison_type
            ):
                survivors.append(addr)
            self._previous[addr] = current
        self._addresses = survivors
        return survivors

    @property
    def addresses(self) -> List[int]:
        return list(self._addresses)

    def _read_value(
        self,
        addr: int,
        byte_width: int,
        value_type: ScanMode,
        byteorder: str,
    ) -> int:
        chunks = [
            self._system.read_memory(addr + offset) for offset in range(byte_width)
        ]
        if byteorder == "little":
            value = sum(b << (8 * i) for i, b in enumerate(chunks))
        else:
            value = sum(b << (8 * (byte_width - 1 - i)) for i, b in enumerate(chunks))
        if value_type == ScanMode.BCD:
            decimal = 0
            for nibble in range(byte_width * 2):
                decimal = decimal * 10 + (
                    (value >> ((byte_width * 2 - 1 - nibble) * 4)) & 0xF
                )
            return decimal
        return value

    def _compare_standard(
        self,
        value: int,
        target: int,
        comparison: StandardComparisonType,
    ) -> bool:
        if comparison == StandardComparisonType.EXACT:
            return value == target
        if comparison == StandardComparisonType.LESS_THAN:
            return value < target
        if comparison == StandardComparisonType.GREATER_THAN:
            return value > target
        if comparison == StandardComparisonType.LESS_THAN_OR_EQUAL:
            return value <= target
        if comparison == StandardComparisonType.GREATER_THAN_OR_EQUAL:
            return value >= target
        raise ValueError(f"Unknown comparison: {comparison}")

    def _compare_dynamic(
        self,
        current: int,
        previous: int,
        new_value: Union[int, None],
        comparison: DynamicComparisonType,
    ) -> bool:
        if comparison == DynamicComparisonType.UNCHANGED:
            return current == previous
        if comparison == DynamicComparisonType.CHANGED:
            return current != previous
        if comparison == DynamicComparisonType.INCREASED:
            return current > previous
        if comparison == DynamicComparisonType.DECREASED:
            return current < previous
        if comparison == DynamicComparisonType.MATCH:
            return current == new_value
        raise ValueError(f"Unknown dynamic comparison: {comparison}")


class GameShark:
    """
    Pure-Python GameShark code applier. Each registered code is a
    `ttvvaaaa` 8-character hex string where `tt` is the type (only
    `01` 8-bit RAM write supported), `vv` is the byte value and
    `aaaa` is the address with the low byte first (so D11E becomes
    1ED1). Codes are applied each frame after the emulator has run,
    so subsequent reads see the cheated value.
    """

    def __init__(self, system: "GameBoy"):
        self._system = system
        self.cheats: Dict[str, Tuple[int, int]] = {}
        self.enabled: bool = True
        self._restore: Dict[int, int] = {}

    def add(self, code: str):
        addr, value = self._parse(code)
        if addr < 0x8000:
            raise ValueError(
                f"GameShark codes targeting ROM are not supported: 0x{addr:04x}"
            )
        if code not in self.cheats:
            self._restore[addr] = self._system.read_memory(addr)
        self.cheats[code] = (addr, value)

    def remove(self, code: str, restore_value: bool = True):
        if code not in self.cheats:
            return
        addr, _value = self.cheats.pop(code)
        if restore_value and addr in self._restore:
            self._system.write_memory(addr, self._restore.pop(addr))

    def clear_all(self, restore_value: bool = True):
        for code in list(self.cheats.keys()):
            self.remove(code, restore_value=restore_value)

    def apply(self):
        if not self.enabled:
            return
        for addr, value in self.cheats.values():
            self._system.write_memory(addr, value)

    def _parse(self, code: str) -> Tuple[int, int]:
        normalized = code.strip().replace(" ", "")
        if len(normalized) != 8:
            raise ValueError(f"GameShark code must be 8 hex chars, got {code!r}")
        try:
            kind = int(normalized[0:2], 16)
            value = int(normalized[2:4], 16)
            low = int(normalized[4:6], 16)
            high = int(normalized[6:8], 16)
        except ValueError as exc:
            raise ValueError(f"GameShark code is not valid hex: {code!r}") from exc
        if kind != 0x01:
            raise ValueError(
                f"Only 8-bit RAM write codes (type 01) are supported, got 0x{kind:02x}"
            )
        return (high << 8) | low, value


def parse_symbols(content: str) -> Dict[str, Tuple[int, int]]:
    """
    Parses the contents of a `.sym` file (RGBDS / no$gmb format)
    and returns a mapping from symbol name to (bank, addr). Lines
    starting with `;` are treated as comments, group headers like
    `[labels]` and `[definitions]` are accepted but otherwise
    ignored.
    """

    symbols: Dict[str, Tuple[int, int]] = {}
    for raw in content.splitlines():
        line = raw.strip()
        if not line or line.startswith(";") or line.startswith("["):
            continue
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        location, name = parts
        if ":" not in location:
            continue
        bank_s, addr_s = location.split(":", 1)
        try:
            bank = int(bank_s, 16)
            addr = int(addr_s, 16)
        except ValueError:
            continue
        symbols[name.strip()] = (bank, addr)
    return symbols
