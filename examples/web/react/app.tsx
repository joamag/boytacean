import React from "react";
import ReactDOM from "react-dom/client";

import { Button } from "./components";

import "./app.css";

export const App = () => {
    const getText = () => "Hello World";
    return <Button text={getText()} />;
};

export const startApp = (element: string) => {
    const root = ReactDOM.createRoot(document.getElementById(element)!);
    root.render(<App />);
};

export default App;
