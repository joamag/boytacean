import React, { useState } from "react";
import ReactDOM from "react-dom/client";

import {
    Button,
    ButtonIncrement,
    ButtonSwitch,
    Info,
    Pair,
    PanelSplit,
    Section,
    Title
} from "./components";

import "./app.css";

export const App = () => {
    const [count, setCount] = useState(0);
    const getText = () => `Hello World ${count}`;
    const onClick = () => setCount(count + 1);
    return (
        <>
            <PanelSplit left={<div>This is the left panel</div>}>
                <Title
                    text="Boytacean"
                    version="0.3.0"
                    versionUrl="https://gitlab.stage.hive.pt/joamag/boytacean/-/blob/master/CHANGELOG.md"
                    iconSrc={require("../res/thunder.png")}
                ></Title>
                <Section>
                    <Button text={getText()} onClick={onClick} />
                    <Button
                        text={getText()}
                        image={require("../res/pause.svg")}
                        imageAlt="tobias"
                        onClick={onClick}
                    />
                    <Info>
                        <Pair
                            key="tobias"
                            name={"Tobias"}
                            value={`Count ${count}`}
                        />
                        <Pair key="matias" name={"Matias"} value={"3"} />
                        <Pair
                            key="button-tobias"
                            name={"Button Increment"}
                            valueNode={
                                <ButtonIncrement
                                    value={200}
                                    delta={100}
                                    min={0}
                                    suffix={"Hz"}
                                />
                            }
                        />
                        <Pair
                            key="button-cpu"
                            name={"Button Switch"}
                            valueNode={
                                <ButtonSwitch
                                    options={["NEO", "CLASSIC"]}
                                    size={"large"}
                                    style={["simple"]}
                                    onChange={(v) => alert(v)}
                                />
                            }
                        />
                    </Info>
                </Section>
            </PanelSplit>
        </>
    );
};

export const startApp = (element: string) => {
    const root = ReactDOM.createRoot(document.getElementById(element)!);
    root.render(<App />);
};

export default App;
