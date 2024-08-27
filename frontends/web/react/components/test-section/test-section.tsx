import { TextInput } from "emukit";
import React, { FC, useMemo } from "react";

import "./test-section.css";

type TestSectionProps = {
    style?: string[];
};

export const TestSection: FC<TestSectionProps> = ({ style = [] }) => {
    const classes = useMemo(
        () => ["test-section", ...style].join(" "),
        [style]
    );
    return (
        <div className={classes}>
            <TextInput size="small" placeholder="XXX-XXX-XXX" />
        </div>
    );
};

export default TestSection;
