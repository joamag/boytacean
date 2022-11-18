import React, { FC, ReactNode } from "react";

import "./panel-tab.css";

type PanelTabProps = {
    tabs?: ReactNode[];
    style?: string[];
};

export const PanelTab: FC<PanelTabProps> = ({
    tabs,
    style = []
}) => {
    const classes = () => ["panel-tab", ...style].join(" ");
    return (
        <div className={classes()}>
        </div>
    );
};

export default PanelTab;
