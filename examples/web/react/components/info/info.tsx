import React, { FC, ReactNode } from "react";

import "./info.css";

type InfoProps = {
    children: ReactNode;
    style?: string[];
};

/**
 * Builds a new info component with the provided pairs components
 * setting the style in accordance with the provided list of strings.
 *
 * An info component is responsible for the management of multiple
 * key to "value" pairs.
 *
 * @param options The multiple options that are going to be used
 * to build the info pairs.
 * @returns The info component with the associated pairs.
 */
export const Info: FC<InfoProps> = ({ children, style = [] }) => {
    const classes = () => ["info", ...style].join(" ");
    return <dl className={classes()}>{children}</dl>;
};

export default Info;
