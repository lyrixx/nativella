<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Menu;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use PHPUnit\Framework\TestCase;

final class MenuTest extends TestCase
{
    public function testSetSendsTheTreeWithoutNulls(): void
    {
        $client = new FakeClient();

        new Menu($client)->set(
            MenuItem::submenu(
                'File',
                MenuItem::link('new', 'New note', '/notes/new', 'CmdOrCtrl+N'),
                MenuItem::checkbox('pin', 'Always on top'),
                MenuItem::separator(),
                MenuItem::role(Role::Quit),
            ),
        );

        $this->assertSame([[
            'type' => 'submenu',
            'label' => 'File',
            'items' => [
                ['type' => 'action', 'id' => 'new', 'label' => 'New note', 'accelerator' => 'CmdOrCtrl+N', 'enabled' => true, 'url' => '/notes/new'],
                ['type' => 'checkbox', 'id' => 'pin', 'label' => 'Always on top', 'checked' => false, 'enabled' => true],
                ['type' => 'separator'],
                ['type' => 'role', 'role' => 'quit'],
            ],
            'enabled' => true,
        ]], $client->calls('menu.set')[0]['payload']['items']);
    }

    public function testUpdateSendsOnlyWhatChanges(): void
    {
        $client = new FakeClient();

        new Menu($client)->update('pin', checked: true);
        new Menu($client)->update('export', label: 'Export 3 notes…', enabled: false);

        $this->assertSame(['id' => 'pin', 'checked' => true], $client->calls('menu.update')[0]['payload']);
        $this->assertSame(['id' => 'export', 'label' => 'Export 3 notes…', 'enabled' => false], $client->calls('menu.update')[1]['payload']);
    }

    public function testTheEditMenuUsesSystemRoles(): void
    {
        $edit = MenuItem::editMenu()->toArray();

        $this->assertSame('Edit', $edit['label']);
        $this->assertContains(['type' => 'role', 'role' => 'copy'], $edit['items']);
    }
}
