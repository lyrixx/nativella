<?php

namespace Nativella\Symfony\Tests\DataCollector;

use Nativella\Client\FakeClient;
use Nativella\Exception\ErrorKind;
use Nativella\Exception\NativeCallException;
use Nativella\Notification;
use Nativella\Secrets;
use Nativella\Symfony\Client\TraceableClient;
use Nativella\Symfony\DataCollector\NativellaDataCollector;
use PHPUnit\Framework\TestCase;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\VarDumper\Cloner\Data;

final class NativellaDataCollectorTest extends TestCase
{
    public function testTheCallsOfTheRequest(): void
    {
        $fake = new FakeClient();
        $fake->fail('notification.send', ErrorKind::Failed, 'No notification server.');
        $client = new TraceableClient($fake);
        try {
            new Notification($client)->send('Saved');
        } catch (NativeCallException) {
        }

        $collector = new NativellaDataCollector($client);
        $collector->collect(new Request(), new Response());

        $this->assertSame(1, $collector->getCallCount());
        $this->assertSame(1, $collector->getErrorCount());
        $this->assertSame('notification.send', $collector->getCalls()[0]['method']);
        $this->assertSame('The native call "notification.send" failed: No notification server.', $collector->getCalls()[0]['error']);
        $this->assertSame(['title' => 'Saved', 'body' => null, 'reference' => null], self::value($collector->getCalls()[0]['payload']));
    }

    public function testTheSecretsStayOutOfTheProfile(): void
    {
        $fake = new FakeClient();
        $fake->respond('secrets.get', ['value' => 'ghp_123']);
        $client = new TraceableClient($fake);
        $secrets = new Secrets($client);
        $secrets->set('github-token', 'ghp_123');
        $secrets->get('github-token');

        $collector = new NativellaDataCollector($client);
        $collector->collect(new Request(), new Response());

        [$set, $get] = $collector->getCalls();
        $this->assertSame(['key' => 'github-token', 'value' => '******'], self::value($set['payload']));
        $this->assertSame(['key' => 'github-token'], self::value($get['payload']));
        $this->assertSame(['value' => '******'], self::value($get['result']));
    }

    private static function value(mixed $data): mixed
    {
        self::assertInstanceOf(Data::class, $data);

        return $data->getValue(true);
    }
}
