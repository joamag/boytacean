import React, { FC } from "react";

import "./tuple.css";

type TupleProps = {
    key: string;
    value?: string;
    style?: string[];
    onKeyClick?: () => void;
    onValueClick?: () => void;
};

export const Tuple: FC<TupleProps> = ({
    key,
    value,
    style = [],
    onKeyClick,
    onValueClick
}) => {
    const classes = () => ["table-entry", ...style].join(" ");
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

export default Tuple;
