import React, { FC } from "react";

import "./toast.css";

type ToastProps = {
    style?: string[];
};

export const Toast: FC<ToastProps> = ({ style = [] }) => {
    const classes = () => ["toast", ...style].join(" ");
    return <div className={classes()}></div>;
};

export default Toast;
