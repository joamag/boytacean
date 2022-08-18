import React, { FC, ReactNode } from "react";

import "./panel-split.css";

type PanelSplitProps = {
    children?: ReactNode;
    left?: ReactNode;
    right?: ReactNode;
    style?: string[];
};

export const PanelSplit: FC<PanelSplitProps> = ({
    children,
    left,
    right,
    style = []
}) => {
    const classes = () => ["panel-split", ...style].join(" ");
    return (
        <div className={classes()}>
            <div className="side-left">{left}</div>
            <div className="side-right">{children || right}</div>
        </div>
    );
};

export default PanelSplit;
