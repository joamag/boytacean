import React, { FC } from "react";
import { AudioGB } from "../audio-gb/audio-gb";
import { RegistersGB } from "../registers-gb/registers-gb";
import { TilesGB } from "../tiles-gb/tiles-gb";
import { GameboyEmulator } from "../../../ts";

import "./debug.css";

type EmulatorProps = {
    emulator: GameboyEmulator;
};

export const DebugVideo: FC<EmulatorProps> = ({ emulator }) => {
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
                <RegistersGB getRegisters={() => emulator.registers} />
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
                <AudioGB getAudioOutput={() => emulator.audioOutput} />
            </div>
        </>
    );
};
