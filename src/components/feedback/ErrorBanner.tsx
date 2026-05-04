type ErrorBannerProps = {
  title: string;
  message: string;
  onDismiss?: () => void;
};

export function ErrorBanner({ title, message, onDismiss }: ErrorBannerProps) {
  return (
    <div className="errorBanner" role="alert">
      <div>
        <strong>{title}</strong>
        <p>{message}</p>
      </div>

      {onDismiss && (
        <button type="button" onClick={onDismiss}>
          Dismiss
        </button>
      )}
    </div>
  );
}
