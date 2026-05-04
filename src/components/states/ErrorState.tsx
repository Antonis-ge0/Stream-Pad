type ErrorStateProps = {
  title: string;
  message: string;
  action?: React.ReactNode;
};

export function ErrorState({ title, message, action }: ErrorStateProps) {
  return (
    <main className="appShell">
      <section className="emptyState">
        <h3>{title}</h3>
        <p>{message}</p>
        {action}
      </section>
    </main>
  );
}
