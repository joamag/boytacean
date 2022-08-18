import React, { FC, ReactNode } from "react";

import "./panel-split.css";

type PanelSplitProps = {
    children: ReactNode;
    style?: string[];
};

export const PanelSplit: FC<PanelSplitProps> = ({ children, style = [] }) => {
    const classes = () => ["panel-split", ...style].join(" ");
    return <></>;
};

export default PanelSplit;
