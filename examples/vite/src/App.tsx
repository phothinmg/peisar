import { Peisar } from "@peisar/peisar-wasm32-wasi";
import content from "../README.md?raw";
const document = new Peisar(content, { fragment: true });

function App() {
  return (
   <div dangerouslySetInnerHTML={{ __html: document.html }} />
  )
}

export default App
