<?php

namespace Nativella\Event;

final readonly class MenuItemClicked
{
    public function __construct(
        public string $id,
        /** The new state of a checkbox, null for other items. */
        public ?bool $checked,
    ) {
    }
}
