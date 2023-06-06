import React from "react";
import { Card } from "antd";

interface DataBoxProps {
    data: string;
}

const DataBox: React.FC<DataBoxProps> = ({ data }) => (
    <Card>
        <p>{data}</p>
    </Card>
);

export default DataBox;
