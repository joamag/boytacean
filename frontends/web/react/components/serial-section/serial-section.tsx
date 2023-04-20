import React, { FC, useEffect, useRef, useState } from "react";
import { ButtonSwitch, Info, Pair, PanelTab } from "emukit";
import { GameboyEmulator, bufferToDataUrl } from "../../../ts";

import "./serial-section.css";

const DEVICE_ICON: { [key: string]: string } = {
    Null: "🛑",
    Logger: "📜",
    Printer: "🖨️"
};

export type LoggerCallback = (data: Uint8Array) => void;
export type PrinterCallback = (imageBuffer: Uint8Array) => void;

type SerialSectionProps = {
    emulator: GameboyEmulator;
    style?: string[];
    onLogger?: (onLoggerData: LoggerCallback) => void;
    onPrinter?: (onPrinterData: PrinterCallback) => void;
};

export const SerialSection: FC<SerialSectionProps> = ({
    emulator,
    style = [],
    onLogger,
    onPrinter
}) => {
    const classes = () => ["serial-section", ...style].join(" ");
    const [loggerData, setLoggerData] = useState<string>();
    const [printerImageUrls, setPrinterImageUrls] = useState<string[]>();
    const loggerDataRef = useRef<string[]>([]);
    const printerDataRef = useRef<string[]>([]);
    const loggerRef = useRef<HTMLDivElement>(null);
    const imagesRef = useRef<HTMLDivElement>(null);

    const onLoggerData = (data: Uint8Array) => {
        const byte = data[0];
        const charByte = String.fromCharCode(byte);
        loggerDataRef.current.push(charByte);
        setLoggerData(loggerDataRef.current.join(""));
    };
    const onPrinterData = (imageBuffer: Uint8Array) => {
        const imageUrl = bufferToDataUrl(imageBuffer, 160);
        printerDataRef.current.unshift(imageUrl);
        setPrinterImageUrls([...printerDataRef.current]);
    };

    useEffect(() => {
        if (loggerRef.current) {
            onLogger && onLogger(onLoggerData);
        }
    }, [loggerRef, loggerRef.current]);

    useEffect(() => {
        if (imagesRef.current) {
            onPrinter && onPrinter(onPrinterData);
        }
    }, [imagesRef, imagesRef.current]);

    const onEngineChange = (option: string) => {
        switch (option) {
            case "Null":
                emulator.loadNullDevice();
                break;

            case "Logger":
                emulator.loadLoggerDevice();
                break;

            case "Printer":
                emulator.loadPrinterDevice();
                break;
        }

        const optionIcon = DEVICE_ICON[option] ?? "";
        emulator.handlers.showToast?.(
            `${optionIcon} ${option} attached to the serial port & active`
        );
    };

    const getTabs = () => {
        return [
            <Info>
                <Pair
                    key="button-device"
                    name={"Device"}
                    valueNode={
                        <ButtonSwitch
                            options={["Null", "Logger", "Printer"]}
                            size={"large"}
                            style={["simple"]}
                            onChange={onEngineChange}
                        />
                    }
                />
                <Pair key="baud-rate" name={"Baud Rate"} value={"1 KB/s"} />
            </Info>,
            <div className="logger" ref={loggerRef}>
                <div className="logger-data">
                    {loggerData || "Logger contents are empty."}
                </div>
            </div>,
            <div className="printer" ref={imagesRef}>
                <div className="printer-lines">
                    {printerImageUrls ? (
                        printerImageUrls.map((url, index) => (
                            <img
                                key={index}
                                className="printer-line"
                                src={url}
                            />
                        ))
                    ) : (
                        <span className="placeholder">
                            Printer contents are empty.
                        </span>
                    )}
                </div>
            </div>
        ];
    };
    const getTabNames = () => {
        return ["Settings", "Logger", "Printer"];
    };
    return (
        <div className={classes()}>
            <PanelTab
                tabs={getTabs()}
                tabNames={getTabNames()}
                selectors={true}
            />
        </div>
    );
};

export default SerialSection;
