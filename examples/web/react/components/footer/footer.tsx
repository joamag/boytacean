import React, { FC, ReactNode } from "react";

import "./footer.css";

type FooterProps = {
    children: ReactNode;
    style?: string[];
};

export const Footer: FC<FooterProps> = ({ children, style = [] }) => {
    const classes = () => ["footer", ...style].join(" ");
    return <div className={classes()}>
        <div className="footer-background"></div>
        <div className="footer-contents">{children}</div>
    </div>
};

export default Footer;
