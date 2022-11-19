import React, { FC, ReactNode, useState } from "react";

import "./panel-tab.css";

type PanelTabProps = {
    tabs: ReactNode[];
    tabNames: string[];
    tabIndex?: number;
    selectors?: boolean;
    style?: string[];
};

export const PanelTab: FC<PanelTabProps> = ({
    tabs,
    tabNames,
    tabIndex = 0,
    selectors = true,
    style = []
}) => {
    const classes = () => ["panel-tab", ...style].join(" ");
    const [tabIndexState, setTabIndexState] = useState(tabIndex);
    return (
        <div className={classes()}>
            {selectors && (
                <div className="tab-selectors">
                    {tabNames.map((tabName, tabIndex) => {
                        const classes = [
                            "tab-selector",
                            tabIndex === tabIndexState ? "selected" : ""
                        ].join(" ");
                        return (
                            <span
                                className={classes}
                                onClick={() => setTabIndexState(tabIndex)}
                            >
                                {tabName}
                            </span>
                        );
                    })}
                </div>
            )}
            <div className="tab-container">{tabs[tabIndexState]}</div>
        </div>
    );
};

export default PanelTab;
