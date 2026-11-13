import { Moon, Sun } from "lucide-react";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { useTheme } from "../lib/theme";
import { useT } from "../lib/i18n";

export function ModeToggle() {
  const { setTheme } = useTheme();
  const t = useT();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" aria-label={t("切换主题")}>
          <Sun className="theme-sun" />
          <Moon className="theme-moon" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuGroup>
          <DropdownMenuItem onSelect={() => setTheme("light")}>
            {t("浅色")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => setTheme("dark")}>
            {t("深色")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => setTheme("system")}>
            {t("跟随系统")}
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
