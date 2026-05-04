type ButtonIconProps = {
  icon?: string | null;
};

export function ButtonIcon({ icon }: ButtonIconProps) {
  if (!icon) return <div className="emptyIcon" />;

  const isImage =
    icon.startsWith("data:") || icon.startsWith("http") || icon.startsWith("/");

  if (isImage) return <img src={icon} alt="" />;

  return <span className="emojiIcon">{icon}</span>;
}
