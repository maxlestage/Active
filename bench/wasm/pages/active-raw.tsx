import { useWasm, wasm } from "active-react";
import { Ready, mount } from "./ready";

const math = wasm<{ add(a: number, b: number): number }>("/raw.wasm");
mount(() => <Ready value={useWasm(math)?.add(2, 3)} />);
