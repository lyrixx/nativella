<?php

namespace Nativella\Symfony\EventListener;

use Nativella\Event\EventFactory;
use Nativella\Runtime;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Event\RequestEvent;
use Symfony\Contracts\EventDispatcher\EventDispatcherInterface;

/**
 * Dispatches the native events the runtime sends (POST /_nativella/event) as
 * Symfony events, named after their class:
 *
 *     #[AsEventListener]
 *     public function __invoke(MenuItemClicked $event): void
 *
 * Every event also comes as a Nativella\Event\NativeEvent, with its name and
 * its raw payload.
 *
 * Only the runtime can reach it, because it sets NATIVELLA_INTERNAL for its
 * own requests, which a page cannot do (headers only become HTTP_* variables).
 */
final readonly class NativeEventListener
{
    public function __construct(
        private EventDispatcherInterface $dispatcher,
    ) {
    }

    public function __invoke(RequestEvent $event): void
    {
        $request = $event->getRequest();

        if (!$event->isMainRequest()
            || Runtime::EVENT_PATH !== $request->getPathInfo()
            || !$request->isMethod('POST')
            || '1' !== $request->server->get('NATIVELLA_INTERNAL')
        ) {
            return;
        }

        foreach (EventFactory::allFromJson($request->getContent()) as $nativeEvent) {
            $this->dispatcher->dispatch($nativeEvent);
        }

        $event->setResponse(new Response(status: Response::HTTP_NO_CONTENT));
    }
}
