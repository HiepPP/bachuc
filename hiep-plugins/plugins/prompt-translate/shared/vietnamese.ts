// Any Vietnamese diacritic counts. Words like "là" share letters with French, but requiring
// Vietnamese-only letters would miss short prompts such as "cái này là gì".
const VIETNAMESE = /[àáảãạăằắẳẵặâầấẩẫậđèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵ]/iu;

export function hasVietnamese(text: string): boolean {
  return VIETNAMESE.test(text);
}
