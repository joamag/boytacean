import React, { FC, ReactNode } from "react";

import "./button-container.css";

type ButtonContainerProps = {
    children: ReactNode;
    style?: string[];
};

export const ButtonContainer: FC<ButtonContainerProps> = ({
    children,
    style = []
}) => {
    const classes = () => ["button-container", ...style].join(" ");
    return <div className={classes()}>{children}</div>;
};

export default ButtonContainer;
