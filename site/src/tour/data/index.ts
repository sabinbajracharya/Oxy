import type { Chapter, Lesson } from '../types';
import { basics } from './basics';
import { functions } from './functions';
import { controlFlow } from './control-flow';
import { dataTypes } from './data-types';
import { collections } from './collections';
import { structsEnums } from './structs-enums';
import { traitsGenerics } from './traits-generics';
import { errorHandling } from './error-handling';
import { modules } from './modules';
import { stdlib } from './stdlib';
import { advanced } from './advanced';

export const CHAPTERS: Chapter[] = [
  basics,
  functions,
  controlFlow,
  dataTypes,
  collections,
  structsEnums,
  traitsGenerics,
  errorHandling,
  modules,
  stdlib,
  advanced,
];

export function findLesson(chapterId: string, lessonId: string): Lesson | undefined {
  const chapter = CHAPTERS.find((c) => c.id === chapterId);
  return chapter?.lessons.find((l) => l.id === lessonId);
}

export function getPrevNext(
  chapterId: string,
  lessonId: string,
): { prev: { chapter: string; lesson: string } | null; next: { chapter: string; lesson: string } | null } {
  const flat: { chapter: string; lesson: string }[] = [];
  for (const ch of CHAPTERS) {
    for (const le of ch.lessons) {
      flat.push({ chapter: ch.id, lesson: le.id });
    }
  }
  const idx = flat.findIndex((e) => e.chapter === chapterId && e.lesson === lessonId);
  return {
    prev: idx > 0 ? flat[idx - 1] : null,
    next: idx < flat.length - 1 ? flat[idx + 1] : null,
  };
}

export function getChapter(chapterId: string): Chapter | undefined {
  return CHAPTERS.find((c) => c.id === chapterId);
}
