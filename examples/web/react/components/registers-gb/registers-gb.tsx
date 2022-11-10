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
        styles: string[] = []
    ) => {
        const classes = ["register", ...styles].join(" ");
        return (
            <div className={classes}>
                <span className="register-key">{key}</span>
                <span className="register-value">0x{value.toString(16)}</span>
            </div>
        );
    };
    return (
        <div className={classes()}>
            <div className="section">
                <h4>CPU</h4>
                {renderRegister("PC", registers.pc as number)}
                {renderRegister("SP", registers.sp as number)}
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
