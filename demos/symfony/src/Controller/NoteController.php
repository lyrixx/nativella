<?php

namespace App\Controller;

use App\Entity\Note;
use App\Form\NoteData;
use App\Form\NoteType;
use App\Native\Desktop;
use App\Note\NoteActions;
use App\Repository\NoteRepository;
use Doctrine\ORM\EntityManagerInterface;
use Nativella\BarcodeScanner;
use Nativella\Clipboard;
use Nativella\Exception\NativeCallException;
use Nativella\Geolocation;
use Nativella\Geolocation\Accuracy;
use Nativella\Menu;
use Nativella\Menu\MenuItem;
use Nativella\Runtime;
use Nativella\Window;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Attribute\MapQueryParameter;
use Symfony\Component\Routing\Attribute\Route;
use Symfony\Component\Routing\Requirement\Requirement;
use Twig\Extra\Markdown\MarkdownInterface;

final class NoteController extends AbstractController
{
    public function __construct(
        private readonly NoteActions $actions,
        private readonly Desktop $desktop,
        private readonly EntityManagerInterface $entityManager,
        private readonly NoteRepository $notes,
    ) {
    }

    #[Route('/', name: 'note_home', methods: ['GET'])]
    public function home(): Response
    {
        $latest = $this->notes->findLatest();
        if (null !== $latest) {
            return $this->redirectToRoute('note_show', ['id' => $latest->getId()]);
        }

        return $this->render('note/empty.html.twig', ['notes' => []]);
    }

    #[Route('/notes', name: 'note_search', methods: ['GET'])]
    public function search(#[MapQueryParameter] string $q = ''): Response
    {
        return $this->render('note/search.html.twig', [
            'notes' => $this->notes->findForSidebar($q),
            'q' => $q,
        ]);
    }

    #[Route('/notes/new', name: 'note_new', methods: ['GET', 'POST'])]
    public function new(Request $request): Response
    {
        $form = $this->createForm(NoteType::class, new NoteData())->handleRequest($request);

        if ($form->isSubmitted() && $form->isValid()) {
            $data = $form->getData();
            $note = new Note($data->title, $data->body);
            $this->notes->add($note);
            $this->entityManager->flush();
            $this->refreshTray();

            return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
        }

        return $this->render('note/form.html.twig', [
            'form' => $form,
            'note' => null,
            'notes' => $this->notes->findForSidebar(),
        ]);
    }

    #[Route('/notes/{id}', name: 'note_show', requirements: ['id' => Requirement::DIGITS], methods: ['GET'])]
    public function show(Note $note, Window $window): Response
    {
        if (Runtime::isNative()) {
            $window->setTitle($note->getTitle().' — Notes');
        }

        return $this->render('note/show.html.twig', [
            'note' => $note,
            'notes' => $this->notes->findForSidebar(),
        ]);
    }

    #[Route('/notes/{id}/edit', name: 'note_edit', requirements: ['id' => Requirement::DIGITS], methods: ['GET', 'POST'])]
    public function edit(Note $note, Request $request): Response
    {
        $form = $this->createForm(NoteType::class, NoteData::fromNote($note))->handleRequest($request);

        if ($form->isSubmitted() && $form->isValid()) {
            $data = $form->getData();
            $note->edit($data->title, $data->body);
            $this->entityManager->flush();

            return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
        }

        return $this->render('note/form.html.twig', [
            'form' => $form,
            'note' => $note,
            'notes' => $this->notes->findForSidebar(),
        ]);
    }

    #[Route('/notes/{id}/delete', name: 'note_delete', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function delete(Note $note): Response
    {
        // Outside Nativella, the page asked with confirm() already.
        if (Runtime::isNative()) {
            if (!$this->actions->delete($note)) {
                return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
            }
        } else {
            $this->notes->remove($note);
        }

        $this->entityManager->flush();
        $this->refreshTray();
        $this->addFlash('success', \sprintf('"%s" was deleted.', $note->getTitle()));

        return $this->redirectToRoute('note_home');
    }

    #[Route('/notes/{id}/pin', name: 'note_pin', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function pin(Note $note): Response
    {
        if ($note->isPinned()) {
            $note->unpin();
        } else {
            $note->pin();
        }
        $this->entityManager->flush();

        return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
    }

    #[Route('/notes/{id}/location', name: 'note_location', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function location(Note $note, Geolocation $geolocation): Response
    {
        if (!Runtime::isMobile()) {
            throw $this->createNotFoundException('Only phones know where they are.');
        }

        try {
            // A note is written somewhere precise: GPS, if the phone has one.
            $position = $geolocation->currentPosition(Accuracy::High);
        } catch (NativeCallException $exception) {
            // Location turned off, or no fix: the message of the phone says which.
            $this->addFlash('success', \sprintf('Notes cannot find where you are: %s', $exception->getPrevious()?->getMessage()));

            return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
        }
        if (null === $position) {
            $this->addFlash('success', 'Notes may not know where you are: allow it in the settings of the phone.');
        } else {
            $note->addLocation($position->latitude, $position->longitude);
            $this->entityManager->flush();
            $this->addFlash('success', 'Your location was added to the note.');
        }

        return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
    }

    #[Route('/notes/{id}/scan', name: 'note_scan', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function scan(Note $note, BarcodeScanner $barcodeScanner): Response
    {
        if (!Runtime::isMobile()) {
            throw $this->createNotFoundException('Only phones have a barcode scanner.');
        }

        try {
            $barcode = $barcodeScanner->scan(cancelLabel: 'Cancel');
        } catch (NativeCallException $exception) {
            // The camera failed, or saw no code: the message of the phone says which.
            $this->addFlash('success', \sprintf('Notes cannot read a code: %s', $exception->getPrevious()?->getMessage()));

            return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
        }
        if (null === $barcode) {
            $this->addFlash('success', 'No code was added to the note.');
        } else {
            $note->addCode($barcode->content);
            $this->entityManager->flush();
            $this->addFlash('success', 'The code was added to the note.');
        }

        return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
    }

    #[Route('/notes/{id}/copy', name: 'note_copy', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function copy(Note $note, Clipboard $clipboard, #[Autowire(service: 'twig.markdown.default')] MarkdownInterface $markdown, Request $request): Response
    {
        if ('html' === $request->getPayload()->getString('format')) {
            $clipboard->writeHtml($markdown->convert($note->getBody()), $note->getBody());
            $this->addFlash('success', 'Copied as rich text: paste it in a mail or a document.');
        } else {
            $clipboard->writeText($note->toMarkdown());
            $this->addFlash('success', 'Copied as Markdown.');
        }

        return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
    }

    #[Route('/notes/{id}/export', name: 'note_export', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function export(Note $note): Response
    {
        $path = $this->actions->export($note);
        if (null !== $path) {
            $this->addFlash('success', \sprintf('Exported to %s.', $path));
        }

        return $this->redirectToRoute('note_show', ['id' => $note->getId()]);
    }

    #[Route('/notes/import', name: 'note_import', methods: ['POST'])]
    public function import(): Response
    {
        $result = $this->actions->importWithDialog();
        $this->entityManager->flush();

        if ([] === $result->notes && [] === $result->skipped) {
            return $this->redirectToRoute('note_home');
        }

        $this->refreshTray();
        $this->addFlash('success', $result->summary());

        if ([] === $result->notes) {
            return $this->redirectToRoute('note_home');
        }

        return $this->redirectToRoute('note_show', ['id' => $result->notes[0]->getId()]);
    }

    /**
     * Opens the native context menu of a note, asked by the page on right
     * click. Its items are handled by App\EventListener\MenuListener.
     */
    #[Route('/notes/{id}/context-menu', name: 'note_context_menu', requirements: ['id' => Requirement::DIGITS], methods: ['POST'])]
    public function contextMenu(Note $note, Menu $menu): Response
    {
        $id = $note->getId();

        $menu->popup(
            MenuItem::link("note.open:$id", 'Open', $this->generateUrl('note_show', ['id' => $id])),
            MenuItem::link("note.edit:$id", 'Edit', $this->generateUrl('note_edit', ['id' => $id])),
            MenuItem::separator(),
            MenuItem::action("note.pin:$id", $note->isPinned() ? 'Unpin' : 'Pin to the top'),
            MenuItem::action("note.duplicate:$id", 'Duplicate'),
            MenuItem::action("note.export:$id", 'Export…'),
            MenuItem::separator(),
            MenuItem::action("note.delete:$id", 'Delete…'),
        );

        return new Response(status: Response::HTTP_NO_CONTENT);
    }

    private function refreshTray(): void
    {
        if (Runtime::isNative() && !Runtime::isMobile()) {
            $this->desktop->refreshTray();
        }
    }
}
