import React, { ReactNode, FC } from "react";

import "./paragraph.css";

type ParagraphProps = {
    children?: ReactNode;
    text?: string;
    style?: string[];
};

export const Paragraph: FC<ParagraphProps> = ({
    children,
    text,
    style = []
}) => {
    const classes = () => ["paragraph", ...style].join(" ");
    return <p className={classes()}>{children ?? text}</p>;
};

export default Paragraph;
