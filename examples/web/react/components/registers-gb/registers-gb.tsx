import React, { FC } from "react";

import "./registers-gb.css";

type RegistersGBProps = {
    registers: Record<string, string | number>;
    style?: string[];
};

export const RegistersGB: FC<RegistersGBProps> = ({
    registers,
    style = []
}) => {
    const classes = () => ["registers-gb", ...style].join(" ");
    const renderRegister = (
        key: string,
        value: number,
        size = 2,
        styles: string[] = []
    ) => {
        const classes = ["register", ...styles].join(" ");
        const valueS = value.toString(16).toUpperCase().padStart(size, "0");
        return (
            <div className={classes}>
                <span className="register-key">{key}</span>
                <span className="register-value">0x{valueS}</span>
            </div>
        );
    };
    return (
        <div className={classes()}>
            <div className="section">
                <h4>CPU</h4>
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
                {renderRegister("LY", registers.l as number)}
                {renderRegister("LYC", registers.l as number)}
            </div>
        </div>
    );
};

export default RegistersGB;
