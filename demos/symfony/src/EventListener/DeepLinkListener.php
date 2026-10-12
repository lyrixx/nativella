<?php

namespace App\EventListener;

use App\Native\Desktop;
use App\Repository\NoteRepository;
use Nativella\Event\AppUrlsOpened;
use Nativella\Window;
use Psr\Log\LoggerInterface;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * nativella-notes://notes/42 opens the note 42 (see "deepLinks" in
 * composer.json), from a browser, a mail or `xdg-open` in a terminal.
 */
#[AsEventListener]
final readonly class DeepLinkListener
{
    public function __construct(
        private Desktop $desktop,
        private LoggerInterface $logger,
        private NoteRepository $notes,
        private Window $window,
    ) {
    }

    public function __invoke(AppUrlsOpened $event): void
    {
        // A link to several notes opens the last one.
        $url = $event->urls[array_key_last($event->urls)];
        if (!preg_match('#^nativella-notes://notes/(?<id>\d+)/?$#', $url, $matches)) {
            $this->logger->warning('Ignored the link "{url}": it names no note.', ['url' => $url]);

            return;
        }

        if (null === $this->notes->find((int) $matches['id'])) {
            $this->logger->warning('The note of the link "{url}" does not exist.', ['url' => $url]);

            return;
        }

        $this->desktop->showMainWindow();
        $this->window->navigate('/notes/'.$matches['id'], Desktop::MAIN_WINDOW);
    }
}
