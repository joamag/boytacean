import React, { FC, useEffect, useMemo, useState } from "react";
import { GameBoySpeed } from "../../../lib/boytacean";

import "./registers-gb.css";

type RegistersGBProps = {
    getRegisters: () => Record<string, string | number>;
    getSpeed?: () => GameBoySpeed;
    interval?: number;
    style?: string[];
};

export const RegistersGB: FC<RegistersGBProps> = ({
    getRegisters,
    getSpeed = () => GameBoySpeed.Normal,
    interval = 50,
    style = []
}) => {
    const classes = useMemo(
        () => ["registers-gb", ...style].join(" "),
        [style]
    );
    const [registers, setRegisters] = useState<Record<string, string | number>>(
        {}
    );
    const [speed, setSpeed] = useState<GameBoySpeed>(GameBoySpeed.Normal);
    useEffect(
        () => {
            const updateValues = () => {
                const registers = getRegisters();
                const speed = getSpeed();
                setRegisters(registers);
                setSpeed(speed);
            };

            updateValues();

            const updateInterval = setInterval(() => updateValues(), interval);
            return () => {
                clearInterval(updateInterval);
            };
        },
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [interval]
    );
    const renderRegister = (
        key: string,
        value?: number,
        size = 2,
        styles: string[] = []
    ) => {
        const classes = () => ["register", ...styles].join(" ");
        const valueS =
            value?.toString(16).toUpperCase().padStart(size, "0") ?? value;
        return (
            <div className={classes()}>
                <span className="register-key">{key}</span>
                <span className="register-value">
                    {valueS ? `0x${valueS}` : "-"}
                </span>
            </div>
        );
    };
    return (
        <div className={classes}>
            <div className="section">
                <h4>CPU {speed == GameBoySpeed.Double ? "2x" : ""}</h4>
                {renderRegister("PC", registers.pc as number, 4)}
                {renderRegister("SP", registers.sp as number, 4)}
                {renderRegister("A", registers.a as number)}
                {renderRegister("B", registers.b as number)}
                {renderRegister("C", registers.c as number)}
                {renderRegister("D", registers.d as number)}
                {renderRegister("E", registers.e as number)}
                {renderRegister("H", registers.h as number)}
                {renderRegister("L", registers.l as number)}
            </div>
            <div className="section">
                <h4>PPU</h4>
                {renderRegister("SCY", registers.scy as number)}
                {renderRegister("SCX", registers.scx as number)}
                {renderRegister("WY", registers.wy as number)}
                {renderRegister("WX", registers.wx as number)}
                {renderRegister("LY", registers.ly as number)}
                {renderRegister("LYC", registers.lyc as number)}
            </div>
        </div>
    );
};

export default RegistersGB;
