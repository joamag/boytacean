import React, { ReactNode, FC } from "react";

import "./link.css";

type LinkProps = {
    children?: ReactNode;
    text?: string;
    href?: string;
    target?: string;
    style?: string[];
};

export const Link: FC<LinkProps> = ({
    children,
    text,
    href,
    target,
    style = []
}) => {
    const classes = () => ["link", ...style].join(" ");
    return (
        <a className={classes()} href={href} target={target}>
            {children ?? text}
        </a>
    );
};

export default Link;
