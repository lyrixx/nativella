<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Nativella;
use PHPUnit\Framework\TestCase;

final class NativellaTest extends TestCase
{
    public function testOutsideTheRuntimeTheApplicationIsNotNative(): void
    {
        $this->assertFalse(Nativella::create()->isNative());
    }

    public function testUnderTheFakeTheApplicationBehavesAsInNativella(): void
    {
        $this->assertTrue(new Nativella(new FakeClient())->isNative());
    }
}
