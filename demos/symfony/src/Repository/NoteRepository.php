<?php

namespace App\Repository;

use App\Entity\Note;
use Doctrine\Bundle\DoctrineBundle\Repository\ServiceEntityRepository;
use Doctrine\Persistence\ManagerRegistry;

/**
 * @extends ServiceEntityRepository<Note>
 */
class NoteRepository extends ServiceEntityRepository
{
    public function __construct(ManagerRegistry $registry)
    {
        parent::__construct($registry, Note::class);
    }

    public function add(Note $note): void
    {
        $this->getEntityManager()->persist($note);
    }

    public function remove(Note $note): void
    {
        $this->getEntityManager()->remove($note);
    }

    /**
     * @return list<Note> pinned notes first, then the most recently updated
     */
    public function findForSidebar(?string $search = null): array
    {
        $query = $this->createQueryBuilder('note')
            ->orderBy('note.pinned', 'DESC')
            ->addOrderBy('note.updatedAt', 'DESC');

        if (null !== $search && '' !== trim($search)) {
            $query
                ->andWhere('LOWER(note.title) LIKE :search OR LOWER(note.body) LIKE :search')
                ->setParameter('search', '%'.addcslashes(mb_strtolower(trim($search)), '%_').'%');
        }

        return $query->getQuery()->getResult();
    }

    public function findLatest(): ?Note
    {
        return $this->findOneBy([], ['pinned' => 'DESC', 'updatedAt' => 'DESC']);
    }
}
