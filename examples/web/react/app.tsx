import React, { useState } from "react";
import ReactDOM from "react-dom/client";

import { Button, ButtonSwitch, Info, Pair } from "./components";

import "./app.css";

export const App = () => {
    const [count, setCount] = useState(0);
    const getText = () => `Hello World ${count}`;
    const onClick = () => setCount(count + 1);
    const pairs = () => [
        <Pair key="tobias" name={"Tobias"} value={`Count ${count}`} />,
        <Pair key="matias" name={"Matias"} value={"3"} />,
        <Pair
            key="button"
            name={"Button"}
            valueNode={
                <ButtonSwitch
                    options={["NEO", "CLASSIC"]}
                    size={"large"}
                    style={["simple"]}
                    onChange={(v) => alert(v)}
                />
            }
        />
    ];
    return (
        <>
            <Button text={getText()} onClick={onClick} />
            <Info pairs={pairs()} />
        </>
    );
};

export const startApp = (element: string) => {
    const root = ReactDOM.createRoot(document.getElementById(element)!);
    root.render(<App />);
};

export default App;
