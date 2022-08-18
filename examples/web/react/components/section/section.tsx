import React, { FC, ReactNode } from "react";

import "./section.css";

type SectionProps = {
    children: ReactNode;
    separator?: boolean;
    style?: string[];
};

export const Section: FC<SectionProps> = ({
    children,
    separator = true,
    style = []
}) => {
    const classes = () => ["section", ...style].join(" ");
    return (
        <div className={classes()}>
            {separator && <div className="separator"></div>}
            <div className="section-contents">{children}</div>
        </div>
    );
};

export default Section;
