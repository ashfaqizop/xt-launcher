export const toast = $state({ text: '', id: 0 });

let timer: ReturnType<typeof setTimeout> | undefined;

export function showToast(text: string, ms = 2800) {
  toast.text = text;
  toast.id++;
  clearTimeout(timer);
  timer = setTimeout(() => (toast.text = ''), ms);
}
