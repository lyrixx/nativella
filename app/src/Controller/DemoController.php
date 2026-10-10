<?php

namespace App\Controller;

use App\Native\Native;
use App\Native\NativeEventLog;
use App\Native\WindowTitle;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapRequestPayload;
use Symfony\Component\Routing\Attribute\Route;

/**
 * One page that exercises the runtime: forms (POST bodies), the session
 * (cookies), redirects, and calls into native code.
 */
final class DemoController extends AbstractController
{
    public function __construct(
        private readonly Native $native,
        private readonly NativeEventLog $eventLog,
    ) {
    }

    #[Route('/', name: 'demo', methods: ['GET'])]
    public function index(Request $request): Response
    {
        $session = $request->getSession();
        $session->set('visits', $session->get('visits', 0) + 1);

        return $this->render('demo/index.html.twig', [
            'events' => $this->eventLog->latest(10),
            'msToController' => (microtime(true) - $request->server->get('REQUEST_TIME_FLOAT')) * 1000,
            'native' => $this->native->isAvailable(),
            'sapi' => \PHP_SAPI,
            'visits' => $session->get('visits'),
        ]);
    }

    #[Route('/native/title', name: 'demo_title', methods: ['POST'])]
    public function title(#[MapRequestPayload] WindowTitle $windowTitle): Response
    {
        $this->native->call('window.set_title', ['title' => $windowTitle->title]);
        $this->addFlash('success', \sprintf('Window title set to "%s".', $windowTitle->title));

        return $this->redirectToRoute('demo');
    }

    #[Route('/native/dialog', name: 'demo_dialog', methods: ['POST'])]
    public function dialog(): Response
    {
        $result = $this->native->call('dialog.ask', [
            'title' => 'native-php',
            'message' => 'This dialog was opened by PHP. Do you like it?',
        ]);
        $this->addFlash('success', $result['answer'] ? 'You answered yes.' : 'You answered no.');

        return $this->redirectToRoute('demo');
    }

    #[Route('/native/pick-file', name: 'demo_pick_file', methods: ['POST'])]
    public function pickFile(): Response
    {
        $result = $this->native->call('dialog.pick_file');

        if (null === $result['path']) {
            $this->addFlash('success', 'No file picked.');
        } else {
            $file = new \SplFileInfo($result['path']);
            $this->addFlash('success', \sprintf('PHP reads "%s": %d bytes.', $file->getFilename(), $file->getSize()));
        }

        return $this->redirectToRoute('demo');
    }

    #[Route('/native/notify', name: 'demo_notify', methods: ['POST'])]
    public function notify(): Response
    {
        $this->native->call('notification.show', [
            'title' => 'Hello from PHP',
            'body' => \sprintf('Sent by PHP %s through the %s SAPI.', \PHP_VERSION, \PHP_SAPI),
        ]);

        return $this->redirectToRoute('demo');
    }
}
