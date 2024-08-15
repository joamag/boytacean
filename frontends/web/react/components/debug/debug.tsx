import React, { FC, useEffect, useState } from "react";
import { ButtonSwitch, Info, Pair } from "emukit";
import { AudioGB } from "../audio-gb/audio-gb";
import { RegistersGB } from "../registers-gb/registers-gb";
import { TilesGB } from "../tiles-gb/tiles-gb";
import { GameboyEmulator } from "../../../ts";

import "./debug.css";

type EmulatorProps = {
    emulator: GameboyEmulator;
};

export const DebugGeneral: FC<EmulatorProps> = ({ emulator }) => {
    return (
        <>
            {emulator.getTile && (
                <div
                    style={{
                        display: "inline-block",
                        verticalAlign: "top",
                        marginRight: 32,
                        width: 256
                    }}
                >
                    <h3>VRAM Tiles</h3>
                    <TilesGB
                        getTile={(index) =>
                            emulator.getTile
                                ? emulator.getTile(index)
                                : new Uint8Array()
                        }
                        tileCount={384}
                        width={"100%"}
                        contentBox={false}
                    />
                </div>
            )}
            <div
                style={{
                    display: "inline-block",
                    verticalAlign: "top"
                }}
            >
                <h3>Registers</h3>
                <RegistersGB
                    getRegisters={() => emulator.registers}
                    getSpeed={() => emulator.speed}
                />
            </div>
        </>
    );
};

export const DebugAudio: FC<EmulatorProps> = ({ emulator }) => {
    return (
        <>
            <div
                style={{
                    display: "inline-block",
                    verticalAlign: "top"
                }}
            >
                <h3>Audio Waveform</h3>
                <AudioGB
                    emulator={emulator}
                    getAudioOutput={() => emulator.audioOutput}
                />
            </div>
        </>
    );
};

export const DebugSettings: FC<EmulatorProps> = ({ emulator }) => {
    return (
        <>
            <DebugSettingsContent emulator={emulator}></DebugSettingsContent>
        </>
    );
};

const DebugSettingsContent: FC<EmulatorProps> = ({ emulator }) => {
    const [updated, setUpdated] = useState(Date.now());

    useEffect(() => {
        const onAudioState = () => {
            setUpdated(Date.now());
        };
        emulator.bind("audio-state", onAudioState);
        return () => {
            emulator.unbind("audio-state", onAudioState);
        };
    }, [emulator]);

    const onPpuChange = (option: string) => {
        emulator.instance?.set_ppu_enabled(option === "on");
    };

    const onApuChange = (option: string) => {
        emulator.instance?.set_apu_enabled(option === "on");
    };

    const onDmaChange = (option: string) => {
        emulator.instance?.set_dma_enabled(option === "on");
    };

    const onTimerChange = (option: string) => {
        emulator.instance?.set_timer_enabled(option === "on");
    };

    const onSerialChange = (option: string) => {
        emulator.instance?.set_serial_enabled(option === "on");
    };

    return (
        <Info key={updated}>
            <Pair
                name={"PPU"}
                valueNode={
                    <ButtonSwitch
                        options={["on", "off"]}
                        value={emulator.instance?.ppu_enabled() ? "on" : "off"}
                        uppercase={true}
                        size={"large"}
                        style={["simple"]}
                        onChange={onPpuChange}
                    />
                }
            />
            <Pair
                name={"APU"}
                valueNode={
                    <ButtonSwitch
                        options={["on", "off"]}
                        value={emulator.instance?.apu_enabled() ? "on" : "off"}
                        uppercase={true}
                        size={"large"}
                        style={["simple"]}
                        onChange={onApuChange}
                    />
                }
            />
            <Pair
                name={"DMA"}
                valueNode={
                    <ButtonSwitch
                        options={["on", "off"]}
                        value={emulator.instance?.dma_enabled() ? "on" : "off"}
                        uppercase={true}
                        size={"large"}
                        style={["simple"]}
                        onChange={onDmaChange}
                    />
                }
            />
            <Pair
                name={"Timer"}
                valueNode={
                    <ButtonSwitch
                        options={["on", "off"]}
                        value={
                            emulator.instance?.timer_enabled() ? "on" : "off"
                        }
                        uppercase={true}
                        size={"large"}
                        style={["simple"]}
                        onChange={onTimerChange}
                    />
                }
            />
            <Pair
                name={"Serial"}
                valueNode={
                    <ButtonSwitch
                        options={["on", "off"]}
                        value={
                            emulator.instance?.serial_enabled() ? "on" : "off"
                        }
                        uppercase={true}
                        size={"large"}
                        style={["simple"]}
                        onChange={onSerialChange}
                    />
                }
            />
        </Info>
    );
};
