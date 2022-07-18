import React, { FC, ReactNode } from "react";

import "./info.css";

type InfoProps = {
    pairs?: ReactNode[];
    style?: string[];
};

export const Info: FC<InfoProps> = ({ pairs = [], style = [] }) => {
    const classes = () => ["info", ...style].join(" ");
    return <dl className={classes()}>{pairs}</dl>;
};

export default Info;
