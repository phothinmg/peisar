import Readme, { html } from "../README.md";

export function App() {
  return (
    <main>
      {/* Renders as a React component */}
      <Readme className="markdown-body" />
    </main>
  );
}

export default App;
