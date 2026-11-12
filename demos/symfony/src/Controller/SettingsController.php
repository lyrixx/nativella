<?php

namespace App\Controller;

use App\Native\Desktop;
use App\Repository\NoteRepository;
use Nativella\App;
use Nativella\App\Directory;
use Nativella\Nativella;
use Nativella\Runtime;
use Nativella\Screen;
use Nativella\Shell;
use Nativella\System;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Security\Http\Attribute\IsCsrfTokenValid;

final class SettingsController extends AbstractController
{
    public function __construct(
        private readonly App $app,
        private readonly Desktop $desktop,
        private readonly Nativella $nativella,
    ) {
    }

    #[Route('/settings', name: 'settings', methods: ['GET'])]
    public function index(NoteRepository $notes, Screen $screen, System $system): Response
    {
        $native = $this->nativella->isNative();

        return $this->render('settings/index.html.twig', [
            'alwaysOnTop' => $native && $this->desktop->isAlwaysOnTop(),
            'appInfo' => $native ? $this->app->info() : null,
            'autostart' => $native && !Runtime::isMobile() && $this->app->isAutostartEnabled(),
            'dataDir' => Runtime::dataDir(),
            'displays' => $native ? $screen->all() : [],
            'notes' => $notes->findForSidebar(),
            'shortcut' => Desktop::QUICK_NOTE_SHORTCUT,
            'system' => $native ? $system->info() : null,
            'theme' => $native ? $system->theme() : null,
        ]);
    }

    #[Route('/settings', name: 'settings_save', methods: ['POST'])]
    #[IsCsrfTokenValid('submit')]
    public function save(Request $request): Response
    {
        if (!$this->nativella->isNative() || Runtime::isMobile()) {
            throw $this->createNotFoundException('Only computers start an application at login, or keep a window above the others.');
        }

        $payload = $request->getPayload();

        if ($payload->getBoolean('autostart')) {
            $this->app->enableAutostart();
        } else {
            $this->app->disableAutostart();
        }
        $this->desktop->setAlwaysOnTop($payload->getBoolean('alwaysOnTop'));

        $this->addFlash('success', 'Settings saved.');

        return $this->redirectToRoute('settings');
    }

    #[Route('/settings/open-data-dir', name: 'settings_open_data_dir', methods: ['POST'])]
    #[IsCsrfTokenValid('submit')]
    public function openDataDir(Shell $shell): Response
    {
        if (!$this->nativella->isNative()) {
            throw $this->createNotFoundException('Only Nativella has a data directory.');
        }

        $shell->openPath($this->app->path(Directory::AppData));

        return $this->redirectToRoute('settings');
    }
}
