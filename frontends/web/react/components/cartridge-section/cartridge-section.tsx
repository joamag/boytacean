import { Button, Info, Pair } from "emukit";
import React, { FC, useMemo, useState } from "react";

import {
    CartridgeInfo,
    CartridgeSave,
    GameboyEmulator,
    requestCartridgePort
} from "../../../ts";

const SAVE_DESCRIPTION: Record<CartridgeSave, string> = {
    [CartridgeSave.Cartridge]: "From the cartridge",
    [CartridgeSave.Storage]: "From the browser (kept)",
    [CartridgeSave.None]: "None"
};

type CartridgeSectionProps = {
    emulator: GameboyEmulator;
    style?: string[];
};

export const CartridgeSection: FC<CartridgeSectionProps> = ({
    emulator,
    style = []
}) => {
    const classes = useMemo(
        () => ["cartridge-section", ...style].join(" "),
        [style]
    );
    const [percentage, setPercentage] = useState<number>();
    const [info, setInfo] = useState<CartridgeInfo>();

    const onLoadClick = async () => {
        if (percentage !== undefined) return;
        try {
            const port = await requestCartridgePort();
            setPercentage(0);

            // the progress is kept as an integer percentage, so that
            // the many progress updates of a read are mostly skipped
            const info = await emulator.loadCartridge(port, (progress) =>
                setPercentage(Math.floor(progress * 100))
            );
            setInfo(info);
            emulator.handlers.showToast?.(
                `🎮 ${info.title} loaded from the cartridge`
            );
        } catch (err) {
            // closing the port selection without choosing a port is
            // not an error, there's nothing to be reported
            if ((err as Error).name !== "NotFoundError") {
                emulator.handlers.showToast?.(
                    `${(err as Error).message}`,
                    true
                );
            }
        } finally {
            setPercentage(undefined);
        }
    };

    return (
        <div className={classes}>
            <Info>
                <Pair
                    key="device"
                    name={"Device"}
                    value={info?.device ?? "GBxCart RW"}
                />
                <Pair key="game" name={"Game"} value={info?.title ?? "-"} />
                <Pair
                    key="save"
                    name={"Save"}
                    value={info ? SAVE_DESCRIPTION[info.save] : "-"}
                />
                <Pair
                    key="checksum"
                    name={"Checksum"}
                    value={
                        info ? (info.checksumValid ? "Valid" : "Invalid") : "-"
                    }
                />
            </Info>
            <Button
                text={
                    percentage === undefined
                        ? "Load from Cartridge"
                        : `Reading ${percentage}%`
                }
                enabled={percentage === undefined}
                size={"large"}
                style={["simple"]}
                onClick={onLoadClick}
            />
        </div>
    );
};

export default CartridgeSection;
