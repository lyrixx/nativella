<?php

namespace App\Controller;

use App\Native\NativeEvent;
use App\Native\NativeEventLog;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Contracts\EventDispatcher\EventDispatcherInterface;

/**
 * Receives the events the runtime sends from the native side.
 *
 * Only the runtime can reach it: it sets NATIVE_PHP_INTERNAL in $_SERVER for
 * its own requests, and a webview request has no way to set a server
 * variable (its headers become HTTP_* variables).
 */
final class NativeEventController extends AbstractController
{
    public function __construct(
        private readonly EventDispatcherInterface $dispatcher,
        private readonly NativeEventLog $eventLog,
    ) {
    }

    #[Route('/_native/event', name: 'native_event', methods: ['POST'], condition: "request.server.get('NATIVE_PHP_INTERNAL') === '1'")]
    public function __invoke(#[MapRequestPayload] NativeEvent $event): Response
    {
        $this->eventLog->record($event);
        $this->dispatcher->dispatch($event, 'native.'.$event->name);

        return new Response(status: Response::HTTP_NO_CONTENT);
    }
}
