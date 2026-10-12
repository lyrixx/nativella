<?php

namespace Nativella;

/**
 * Desktop notifications.
 */
final readonly class Notification
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * @param string|null $reference given back by Event\NotificationClicked when the user clicks the notification (Linux only, for now)
     */
    public function send(string $title, ?string $body = null, ?string $reference = null): void
    {
        $this->client->call('notification.send', ['title' => $title, 'body' => $body, 'reference' => $reference]);
    }
}
