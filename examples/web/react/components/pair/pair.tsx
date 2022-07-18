import React, { FC } from "react";

import "./pair.css";

type PairProps = {
    key: string;
    value?: string;
    style?: string[];
    onKeyClick?: () => void;
    onValueClick?: () => void;
};

export const Pair: FC<PairProps> = ({
    key,
    value,
    style = [],
    onKeyClick,
    onValueClick
}) => {
    const classes = () => ["pair", ...style].join(" ");
    const _onKeyClick = () => (onKeyClick ? onKeyClick() : undefined);
    const _onValueClick = () => (onValueClick ? onValueClick() : undefined);
    return (
        <>
            <dt className={classes()} onClick={_onKeyClick}>
                {key}
            </dt>
            <dd className={classes()} onClick={_onValueClick}>
                {value ?? ""}
            </dd>
        </>
    );
};

export default Pair;
