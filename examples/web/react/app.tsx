import React, { useState } from "react";
import ReactDOM from "react-dom/client";

import {
    Button,
    ButtonIncrement,
    ButtonSwitch,
    Info,
    Pair
} from "./components";

import "./app.css";

export const App = () => {
    const [count, setCount] = useState(0);
    const getText = () => `Hello World ${count}`;
    const onClick = () => setCount(count + 1);
    return (
        <>
            <Button text={getText()} onClick={onClick} />
            <Button
                text={getText()}
                image={require("../res/pause.svg")}
                imageAlt="tobias"
                onClick={onClick}
            />
            <Info>
                <Pair key="tobias" name={"Tobias"} value={`Count ${count}`} />
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
        </>
    );
};

export const startApp = (element: string) => {
    const root = ReactDOM.createRoot(document.getElementById(element)!);
    root.render(<App />);
};

export default App;
