<?php

namespace Nativella\Event;

/**
 * The user clicked a notification sent with a reference.
 */
final readonly class NotificationClicked
{
    public function __construct(
        /** As given to Notification::send(). */
        public string $reference,
    ) {
    }
}
