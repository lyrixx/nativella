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
     * Pinned notes first, then the most recently updated. As SQLite's LIKE,
     * the search ignores the case of ASCII letters only.
     *
     * @return list<Note>
     */
    public function findForSidebar(?string $search = null): array
    {
        $query = $this->createQueryBuilder('note')
            ->orderBy('note.pinned', 'DESC')
            ->addOrderBy('note.updatedAt', 'DESC');

        if (null !== $search && '' !== trim($search)) {
            $query
                // LIKE has no escape character unless told: "%" and "_" of the search are meant literally.
                ->andWhere("note.title LIKE :search ESCAPE '\\' OR note.body LIKE :search ESCAPE '\\'")
                ->setParameter('search', '%'.addcslashes(trim($search), '%_\\').'%');
        }

        return $query->getQuery()->getResult();
    }

    public function findLatest(): ?Note
    {
        return $this->findOneBy([], ['pinned' => 'DESC', 'updatedAt' => 'DESC']);
    }
}
