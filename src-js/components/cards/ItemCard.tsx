import type { ShelfItem } from "../../store/shelf";
import { FileCard } from "./FileCard";
import { ImageCard } from "./ImageCard";
import { TextCard } from "./TextCard";
import { UrlCard } from "./UrlCard";

interface ItemCardProps {
  item: ShelfItem;
  onRemove: () => void;
  onCopy: (text: string) => void;
  onHandleMouseDown: (e: React.MouseEvent) => void;
}

export function ItemCard({
  item,
  onRemove,
  onCopy,
  onHandleMouseDown,
}: ItemCardProps) {
  switch (item.kind) {
    case "image":
      return (
        <ImageCard
          item={item}
          onRemove={onRemove}
          onHandleMouseDown={onHandleMouseDown}
        />
      );
    case "text":
      return (
        <TextCard
          item={item}
          onRemove={onRemove}
          onCopy={onCopy}
          onHandleMouseDown={onHandleMouseDown}
        />
      );
    case "url":
      return (
        <UrlCard
          item={item}
          onRemove={onRemove}
          onHandleMouseDown={onHandleMouseDown}
        />
      );
    default:
      return (
        <FileCard
          item={item}
          onRemove={onRemove}
          onCopy={onCopy}
          onHandleMouseDown={onHandleMouseDown}
        />
      );
  }
}
