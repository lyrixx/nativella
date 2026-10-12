<?php

namespace App\Tests;

use App\Entity\Note;
use App\Repository\NoteRepository;
use Doctrine\ORM\EntityManagerInterface;
use Nativella\Client\FakeClient;
use Symfony\Bundle\FrameworkBundle\KernelBrowser;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;

abstract class NotesTestCase extends WebTestCase
{
    protected KernelBrowser $client;

    protected function setUp(): void
    {
        $this->client = static::createClient();
        // The container is rebuilt for every request: keep the same fake.
        $this->client->disableReboot();
        $this->entityManager()->createQuery('DELETE FROM '.Note::class)->execute();
    }

    protected function fakeClient(): FakeClient
    {
        return $this->service(FakeClient::class);
    }

    protected function notes(): NoteRepository
    {
        return $this->service(NoteRepository::class);
    }

    protected function entityManager(): EntityManagerInterface
    {
        return $this->service(EntityManagerInterface::class);
    }

    protected function createNote(string $title = 'Groceries', string $body = "- milk\n- bread"): Note
    {
        $note = new Note($title, $body);
        $this->notes()->add($note);
        $this->entityManager()->flush();

        return $note;
    }

    /**
     * Sends a native event the way the runtime does.
     *
     * @param array<string, mixed> $payload
     */
    protected function sendNativeEvent(string $name, array $payload = []): void
    {
        $this->client->request(
            'POST',
            '/_nativella/event',
            server: ['NATIVELLA_INTERNAL' => '1', 'CONTENT_TYPE' => 'application/json'],
            content: json_encode(['name' => $name, 'payload' => (object) $payload], \JSON_THROW_ON_ERROR),
        );

        $this->assertResponseStatusCodeSame(204);
    }

    /**
     * @return list<string>
     */
    protected function nativeMethods(): array
    {
        return array_column($this->fakeClient()->calls(), 'method');
    }

    /**
     * @template T of object
     *
     * @param class-string<T> $class
     *
     * @return T
     */
    private function service(string $class): object
    {
        $service = static::getContainer()->get($class);
        $this->assertInstanceOf($class, $service);

        return $service;
    }
}
