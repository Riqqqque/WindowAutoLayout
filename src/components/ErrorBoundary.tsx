import { AlertTriangle, RotateCcw } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  failed: boolean;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { failed: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("WindowAutoLayout interface error", error, info.componentStack);
  }

  render() {
    if (!this.state.failed) return this.props.children;

    return (
      <main className="app-frame flex items-center justify-center p-6 text-zinc-100">
        <section className="panel grid max-w-md gap-4 p-5 text-center">
          <AlertTriangle className="mx-auto text-amber-300" size={26} />
          <div>
            <h1 className="section-heading">Interface recovery</h1>
            <p className="mt-2 text-sm text-[#91a0ab]">
              WindowAutoLayout is still running. Reload the interface to reconnect.
            </p>
          </div>
          <button className="button-primary mx-auto" onClick={() => window.location.reload()}>
            <RotateCcw size={15} />
            Reload
          </button>
        </section>
      </main>
    );
  }
}
