import { useWasm } from "react-wasm";
import { Ready, mount } from "./ready";

mount(() => {
  const { data } = useWasm({ url: "/raw.wasm" });
  return <Ready value={(data?.instance.exports.add as any)?.(2, 3)} />;
});
