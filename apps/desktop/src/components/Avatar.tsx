import { createAvatarInitials } from "../domain/chat";

export function Avatar({ title }: { title: string }) {
  return <span className="avatar">{createAvatarInitials(title)}</span>;
}
