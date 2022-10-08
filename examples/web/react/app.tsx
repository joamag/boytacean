import React, { useState } from "react";
import ReactDOM from "react-dom/client";

import {
    Button,
    ButtonIncrement,
    ButtonSwitch,
    Footer,
    Info,
    Link,
    Pair,
    PanelSplit,
    Paragraph,
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
            <Footer>
                Built with ❤️ by <a href="https://joao.me" target="_blank">João Magalhães</a>
            </Footer>
            <PanelSplit left={<div>This is the left panel</div>}>
                <Title
                    text="Boytacean"
                    version="0.3.0"
                    versionUrl="https://gitlab.stage.hive.pt/joamag/boytacean/-/blob/master/CHANGELOG.md"
                    iconSrc={require("../res/thunder.png")}
                ></Title>
                <Section>
                    <Paragraph>
                        This is a{" "}
                        <Link
                            href="https://en.wikipedia.org/wiki/Game_Boy"
                            target="_blank"
                        >
                            Game Boy
                        </Link>{" "}
                        emulator built using the{" "}
                        <Link href="https://www.rust-lang.org" target="_blank">
                            Rust Programming Language
                        </Link>{" "}
                        and is running inside this browser with the help of{" "}
                        <Link href="https://webassembly.org/" target="_blank">
                            WebAssembly
                        </Link>
                        .
                    </Paragraph>
                    <Paragraph>
                        You can check the source code of it at{" "}
                        <Link
                            href="https://gitlab.stage.hive.pt/joamag/boytacean"
                            target="_blank"
                        >
                            GitLab
                        </Link>
                        .
                    </Paragraph>
                    <Paragraph>
                        TIP: Drag and Drop ROM files to the Browser to load the
                        ROM.
                    </Paragraph>
                </Section>
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
    const elementRef = document.getElementById(element);
    if (!elementRef) {
        return;
    }

    const root = ReactDOM.createRoot(elementRef);
    root.render(<App />);
};

export default App;
