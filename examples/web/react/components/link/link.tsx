import React, { FC } from "react";

import "./link.css";

type LinkProps = {
    text: string;
    href?: string;
    target?: string;
    style?: string[];
};

export const Link: FC<LinkProps> = ({ text, href, target, style = [] }) => {
    const classes = () => ["link", ...style].join(" ");
    return (
        <a className={classes()} href={href} target={target}>
            {text}
        </a>
    );
};

export default Link;
