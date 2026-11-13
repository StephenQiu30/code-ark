import { Fragment } from "react";
import type { ReactNode } from "react";
import { ItemGroup } from "./ui/item";
import { Separator } from "./ui/separator";

export function ResourceList<T>({
  items,
  getKey,
  children,
}: {
  items: T[];
  getKey: (item: T) => string;
  children: (item: T) => ReactNode;
}) {
  return (
    <ItemGroup className="gap-0">
      {items.map((item, index) => (
        <Fragment key={getKey(item)}>
          {index > 0 && <Separator />}
          <div role="listitem">{children(item)}</div>
        </Fragment>
      ))}
    </ItemGroup>
  );
}
