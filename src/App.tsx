import React from "react";
import { Button, Input } from "antd";
import "antd/dist/reset.css";
import "./App.css";
import { invoke } from "@tauri-apps/api";
import DataBox from "./DataBox";
// When using the Tauri global script (if not using the npm package)
// Be sure to set `build.withGlobalTauri` in `tauri.conf.json` to true

const ClipboardBroadcast = (message: String) => {
    invoke("clipboard_broadcast", { message });
    console.log(`Broadcast: ${message}`);
};

const startListening = () => {
    const response = invoke("start_liaison");
    response.then((res) => {
        console.log(res);
    });
};

class App extends React.Component {
    state = {
        message: "Hello from React",
        cliplist: [],
        timer: null,
    };

    componentDidMount() {
        this.setState({
            timer: setInterval(() => {
                const response = invoke("clipboard_get");
                response.then((res) =>
                    this.setState({
                        cliplist: res,
                    })
                );
            }, 100),
        });
    }

    componentWillUnmount() {
        if (this.state.timer) {
            clearInterval(this.state.timer);
        }
    }

    render() {
        return (
            <div className="App">
                <Button type="primary" onClick={startListening}>
                    Start
                </Button>
                <Button
                    type="primary"
                    onClick={() => {
                        ClipboardBroadcast(this.state.message);
                    }}
                >
                    Broadcast
                </Button>
                {/* <Button type="primary" onClick={startListening}>
                    Clear
                </Button> */}
                <Input
                    value={this.state.message}
                    onChange={(e) =>
                        this.setState({
                            message: e.target.value,
                        })
                    }
                />
                
                {this.state.cliplist.map((item, index) => {
                    return <DataBox data={item} key={index} />;
                })}
            </div>
        );
    }
}

export default App;
