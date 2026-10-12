<?php

namespace App\Controller;

use App\Native\Desktop;
use App\Repository\NoteRepository;
use Nativella\App;
use Nativella\App\Directory;
use Nativella\Runtime;
use Nativella\Screen;
use Nativella\Shell;
use Nativella\System;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\Routing\Attribute\Route;

final class SettingsController extends AbstractController
{
    public function __construct(
        private readonly App $app,
        private readonly Desktop $desktop,
    ) {
    }

    #[Route('/settings', name: 'settings', methods: ['GET'])]
    public function index(NoteRepository $notes, Screen $screen, System $system): Response
    {
        $native = Runtime::isNative();

        return $this->render('settings/index.html.twig', [
            'alwaysOnTop' => $native && $this->desktop->isAlwaysOnTop(),
            'appInfo' => $native ? $this->app->info() : null,
            'autostart' => $native && !Runtime::isMobile() && $this->app->isAutostartEnabled(),
            'dataDir' => Runtime::dataDir(),
            'notes' => $notes->findForSidebar(),
            'screens' => $native ? $screen->all() : [],
            'shortcut' => Desktop::QUICK_NOTE_SHORTCUT,
            'system' => $native ? $system->info() : null,
            'theme' => $native ? $system->theme() : null,
        ]);
    }

    #[Route('/settings', name: 'settings_save', methods: ['POST'])]
    public function save(Request $request): Response
    {
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
    public function openDataDir(Shell $shell): Response
    {
        $shell->openPath($this->app->path(Directory::AppData));

        return $this->redirectToRoute('settings');
    }
}
