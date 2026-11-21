import assert from "assert";
import { createRequire } from "module";

import { Children, isValidElement, ReactElement, ReactNode } from "react";

import { BoytaceanSystem } from "../../react/boytacean-context";

const requireModule = createRequire(__filename);
const react = requireModule("react") as Record<string, unknown>;
const originalCss = requireModule.extensions[".css"];
requireModule.extensions[".css"] = () => {};
const { BoytaceanGamepad } = requireModule(
    "../../react/boytacean-gamepad"
) as typeof import("../../react/boytacean-gamepad");
if (originalCss) requireModule.extensions[".css"] = originalCss;
else delete requireModule.extensions[".css"];

type Button = {
    children: string;
    onMouseDown: () => void;
    onMouseUp: () => void;
    onMouseLeave: () => void;
    onTouchStart: (event: { preventDefault: () => void }) => void;
    onTouchEnd: (event: { preventDefault: () => void }) => void;
    onTouchCancel: () => void;
};

/**
 * Renders the game pad with a stubbed context, retaining the native
 * button handlers so that input can be exercised without a browser.
 *
 * @param system The system whose controls should be rendered.
 * @returns The rendered buttons and recorded key events.
 */
const renderGamepad = (system: BoytaceanSystem) => {
    const buttons: Button[] = [];
    const events: string[] = [];
    const original = { ...react };
    react.useContext = () => ({
        system,
        press: (key: string) => events.push(`press:${key}`),
        release: (key: string) => events.push(`release:${key}`)
    });
    const visit = (node: ReactNode) => {
        if (!isValidElement(node)) return;
        const element = node as ReactElement<Record<string, unknown>>;
        if (typeof element.type === "function") {
            visit(
                (element.type as (props: Record<string, unknown>) => ReactNode)(
                    element.props
                )
            );
        } else {
            if (element.type === "button")
                buttons.push(element.props as unknown as Button);
            Children.forEach(element.props.children as ReactNode, visit);
        }
    };
    try {
        const element = BoytaceanGamepad({
            className: "custom",
            style: { opacity: 0.5 }
        }) as ReactElement;
        visit(element);
        return { buttons, events, element };
    } finally {
        Object.assign(react, original);
    }
};

describe("BoytaceanGamepad", function () {
    describe("#GamepadButton()", function () {
        it("should be able to press and release shoulder keys with the mouse", () => {
            const { buttons, events } = renderGamepad(
                BoytaceanSystem.GameBoyAdvance
            );
            for (const key of ["L", "R"]) {
                const button = buttons.find(
                    (button) => button.children === key
                )!;
                button.onMouseDown();
                button.onMouseUp();
                button.onMouseDown();
                button.onMouseLeave();
                assert.deepStrictEqual(events.splice(0), [
                    `press:${key}`,
                    `release:${key}`,
                    `press:${key}`,
                    `release:${key}`
                ]);
            }
        });

        it("should be able to release shoulder keys when a touch ends or is cancelled", () => {
            const { buttons, events } = renderGamepad(
                BoytaceanSystem.GameBoyAdvance
            );
            for (const key of ["L", "R"]) {
                let prevented = 0;
                const event = {
                    preventDefault: () => {
                        prevented++;
                    }
                };
                const button = buttons.find(
                    (button) => button.children === key
                )!;
                button.onTouchStart(event);
                button.onTouchEnd(event);
                button.onTouchStart(event);
                button.onTouchCancel();
                assert.strictEqual(prevented, 3);
                assert.deepStrictEqual(events.splice(0), [
                    `press:${key}`,
                    `release:${key}`,
                    `press:${key}`,
                    `release:${key}`
                ]);
            }
        });
    });

    describe("#BoytaceanGamepad()", function () {
        it("should be able to retain the Game Boy controls without shoulder keys", () => {
            const { buttons, element } = renderGamepad(BoytaceanSystem.GameBoy);
            assert.deepStrictEqual(
                buttons.map((button) => button.children),
                ["▲", "◀", "▶", "▼", "B", "A", "SELECT", "START"]
            );
            assert.strictEqual(element.props.className, "gamepad custom");
            assert.deepStrictEqual(element.props.style, { opacity: 0.5 });
        });

        it("should be able to expose both shoulder controls for GBA", () => {
            const { buttons } = renderGamepad(BoytaceanSystem.GameBoyAdvance);
            assert.deepStrictEqual(
                buttons.map((button) => button.children),
                ["L", "R", "▲", "◀", "▶", "▼", "B", "A", "SELECT", "START"]
            );
        });
    });
});
