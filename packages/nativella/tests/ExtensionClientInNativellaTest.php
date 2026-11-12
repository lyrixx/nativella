<?php

namespace Nativella\Tests;

use Nativella\Client\ExtensionClient;
use Nativella\Exception\NativeCallException;
use PHPUnit\Framework\Attributes\RunTestsInSeparateProcesses;
use PHPUnit\Framework\TestCase;

/**
 * As PHP runs in Nativella: nativella_call() exists. A function cannot be
 * removed once defined, hence the separate processes.
 */
#[RunTestsInSeparateProcesses]
final class ExtensionClientInNativellaTest extends TestCase
{
    protected function setUp(): void
    {
        require_once __DIR__.'/Fixtures/nativella_call.php';
        $GLOBALS['nativella_calls'] = [];
        unset($GLOBALS['nativella_failure']);
    }

    public function testItCallsTheRuntime(): void
    {
        $client = new ExtensionClient();

        $this->assertTrue($client->isNative());
        $this->assertSame([], $client->call('clipboard.writeText', ['text' => 'Grüße']));
        $this->assertSame([['method' => 'clipboard.writeText', 'payload' => ['text' => 'Grüße']]], $GLOBALS['nativella_calls']);
    }

    public function testAStringThatIsNotUtf8CannotBeSent(): void
    {
        try {
            new ExtensionClient()->call('clipboard.writeText', ['text' => "Gr\xFC\xDFe"]);
            $this->fail('A NativeCallException was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame('The native call "clipboard.writeText" failed: Malformed UTF-8 characters, possibly incorrectly encoded', $e->getMessage());
            $this->assertInstanceOf(\JsonException::class, $e->getPrevious());
        }

        $this->assertSame([], $GLOBALS['nativella_calls']);
    }

    public function testAFailureOfTheNativeSideNamesTheMethod(): void
    {
        $GLOBALS['nativella_failure'] = 'There is no "settings" window.';

        try {
            new ExtensionClient()->call('window.close', ['window' => 'settings']);
            $this->fail('A NativeCallException was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame('window.close', $e->method);
            $this->assertSame('The native call "window.close" failed: There is no "settings" window.', $e->getMessage());
            $this->assertInstanceOf(\RuntimeException::class, $e->getPrevious());
        }
    }
}
