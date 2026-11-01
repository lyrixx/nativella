<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Screen;
use Nativella\Screen\Display;
use Nativella\Screen\Point;
use Nativella\Screen\Rectangle;
use PHPUnit\Framework\TestCase;

final class ScreenTest extends TestCase
{
    public function testTheDisplaysWithTheirWorkArea(): void
    {
        $client = new FakeClient();
        $client->respond('screen.all', ['displays' => [
            ['name' => 'eDP-1', 'primary' => true, 'x' => 0, 'y' => 0, 'width' => 1280, 'height' => 800, 'scaleFactor' => 2, 'workArea' => ['x' => 0, 'y' => 32, 'width' => 1280, 'height' => 768]],
            ['name' => null, 'primary' => false, 'x' => 1280, 'y' => 0, 'width' => 1920.5, 'height' => 1080, 'scaleFactor' => 1.25, 'workArea' => ['x' => 1280, 'y' => 0, 'width' => 1920.5, 'height' => 1040]],
        ]]);

        $this->assertEquals([
            new Display('eDP-1', true, new Rectangle(0.0, 0.0, 1280.0, 800.0), new Rectangle(0.0, 32.0, 1280.0, 768.0), 2.0),
            new Display(null, false, new Rectangle(1280.0, 0.0, 1920.5, 1080.0), new Rectangle(1280.0, 0.0, 1920.5, 1040.0), 1.25),
        ], new Screen($client)->all());
    }

    public function testThePrimaryDisplay(): void
    {
        $client = new FakeClient();
        $client->respond('screen.primary', ['display' => ['name' => 'eDP-1', 'primary' => true, 'x' => 0, 'y' => 0, 'width' => 1280, 'height' => 800, 'scaleFactor' => 1, 'workArea' => ['x' => 0, 'y' => 0, 'width' => 1280, 'height' => 770]]]);
        $screen = new Screen($client);

        $this->assertSame(770.0, $screen->primary()?->workArea->height);
        // Under Wayland, among others.
        $this->assertNull($screen->primary());
    }

    public function testTheCursor(): void
    {
        $client = new FakeClient();
        $client->respond('screen.cursorPosition', ['x' => 640, 'y' => 400]);

        $this->assertEquals(new Point(640.0, 400.0), new Screen($client)->cursorPosition());
    }
}
