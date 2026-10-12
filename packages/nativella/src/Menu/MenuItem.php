<?php

namespace Nativella\Menu;

/**
 * An entry of the application menu, a context menu or the tray menu.
 *
 * Clicking an action or a checkbox dispatches Event\MenuItemClicked with its
 * id; clicking a link also opens its URL in the focused window.
 */
final readonly class MenuItem
{
    /**
     * @param array<string, mixed> $data
     */
    private function __construct(
        private array $data,
    ) {
    }

    public static function action(string $id, string $label, ?string $accelerator = null, bool $enabled = true): self
    {
        return new self(['type' => 'action', 'id' => $id, 'label' => $label, 'accelerator' => $accelerator, 'enabled' => $enabled]);
    }

    /**
     * An action that opens $url, a path of the application, in the focused window.
     */
    public static function link(string $id, string $label, string $url, ?string $accelerator = null, bool $enabled = true): self
    {
        return new self(['type' => 'action', 'id' => $id, 'label' => $label, 'accelerator' => $accelerator, 'enabled' => $enabled, 'url' => $url]);
    }

    public static function checkbox(string $id, string $label, bool $checked = false, ?string $accelerator = null, bool $enabled = true): self
    {
        return new self(['type' => 'checkbox', 'id' => $id, 'label' => $label, 'checked' => $checked, 'accelerator' => $accelerator, 'enabled' => $enabled]);
    }

    public static function separator(): self
    {
        return new self(['type' => 'separator']);
    }

    public static function submenu(string $label, self ...$items): self
    {
        return new self(['type' => 'submenu', 'label' => $label, 'items' => array_values($items), 'enabled' => true]);
    }

    public static function role(Role $role): self
    {
        return new self(['type' => 'role', 'role' => $role->value]);
    }

    /**
     * The submenu named after the application on macOS: about, hide, quit.
     */
    public static function appMenu(string $name): self
    {
        return self::submenu(
            $name,
            self::role(Role::About),
            self::separator(),
            self::role(Role::Services),
            self::separator(),
            self::role(Role::Hide),
            self::role(Role::HideOthers),
            self::role(Role::ShowAll),
            self::separator(),
            self::role(Role::Quit),
        );
    }

    public static function editMenu(string $label = 'Edit'): self
    {
        return self::submenu(
            $label,
            self::role(Role::Undo),
            self::role(Role::Redo),
            self::separator(),
            self::role(Role::Cut),
            self::role(Role::Copy),
            self::role(Role::Paste),
            self::role(Role::SelectAll),
        );
    }

    public static function windowMenu(string $label = 'Window'): self
    {
        return self::submenu(
            $label,
            self::role(Role::Minimize),
            self::role(Role::Maximize),
            self::role(Role::Fullscreen),
            self::separator(),
            self::role(Role::CloseWindow),
        );
    }

    /**
     * @return array<string, mixed> the entry as the runtime reads it
     */
    public function toArray(): array
    {
        $data = $this->data;
        if (isset($data['items'])) {
            $data['items'] = array_map(static fn (self $item): array => $item->toArray(), $data['items']);
        }

        return array_filter($data, static fn (mixed $value): bool => null !== $value);
    }
}
