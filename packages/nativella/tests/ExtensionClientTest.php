<?php

namespace Nativella\Tests;

use Nativella\Client\ExtensionClient;
use Nativella\Exception\RuntimeUnavailableException;
use PHPUnit\Framework\TestCase;

final class ExtensionClientTest extends TestCase
{
    public function testItFailsOutsideTheRuntime(): void
    {
        $this->expectException(RuntimeUnavailableException::class);
        $this->expectExceptionMessage('Cannot call "window.open": the application does not run in Nativella.');

        new ExtensionClient()->call('window.open');
    }
}
