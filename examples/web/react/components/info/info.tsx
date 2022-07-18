import React, { FC } from "react";

import "./info.css";

type InfoProps = {
    style?: string[];
};

export const Info: FC<InfoProps> = ({ style = [] }) => {
    const classes = () => ["info", ...style].join(" ");
    return <dl className={classes()}></dl>;
};

export default Info;
