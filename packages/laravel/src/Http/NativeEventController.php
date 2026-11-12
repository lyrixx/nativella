<?php

namespace Nativella\Laravel\Http;

use Illuminate\Contracts\Events\Dispatcher;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Nativella\Event\EventFactory;
use Nativella\Runtime;

/**
 * Dispatches the native events the runtime sends (POST /_nativella/event) as
 * Laravel events, named after their class:
 *
 *     public function handle(MenuItemClicked $event): void
 *
 * Every event also comes as a Nativella\Event\NativeEvent, with its name and
 * its raw payload.
 *
 * Only the runtime can reach it, because it sets NATIVELLA_INTERNAL for its
 * own requests, which a page cannot do (headers only become HTTP_* variables).
 */
final readonly class NativeEventController
{
    public function __construct(
        private Dispatcher $events,
    ) {
    }

    public function __invoke(Request $request): Response
    {
        abort_unless(Runtime::isInternalRequest(), Response::HTTP_NOT_FOUND);

        foreach (EventFactory::allFromJson($request->getContent()) as $event) {
            $this->events->dispatch($event);
        }

        return new Response(status: Response::HTTP_NO_CONTENT);
    }
}
