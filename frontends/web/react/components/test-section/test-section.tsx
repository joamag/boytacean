import React, { FC } from "react";
import { TextInput } from "emukit";

import "./test-section.css";

type TestSectionProps = {
    style?: string[];
};

export const TestSection: FC<TestSectionProps> = ({ style = [] }) => {
    const classes = () => ["test-section", ...style].join(" ");
    return (
        <div className={classes()}>
            <TextInput />
        </div>
    );
};

export default TestSection;
