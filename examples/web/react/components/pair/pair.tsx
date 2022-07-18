import React, { FC, ReactNode } from "react";

import "./pair.css";

type PairProps = {
    name: string;
    value?: string;
    valueNode?: ReactNode;
    style?: string[];
    onNameClick?: () => void;
    onValueClick?: () => void;
};

export const Pair: FC<PairProps> = ({
    name,
    value,
    valueNode,
    style = [],
    onNameClick,
    onValueClick
}) => {
    const classes = () => ["pair", ...style].join(" ");
    const _onNameClick = () => (onNameClick ? onNameClick() : undefined);
    const _onValueClick = () => (onValueClick ? onValueClick() : undefined);
    return (
        <>
            <dt className={classes()} onClick={_onNameClick}>
                {name}
            </dt>
            <dd className={classes()} onClick={_onValueClick}>
                {valueNode ?? value ?? ""}
            </dd>
        </>
    );
};

export default Pair;
